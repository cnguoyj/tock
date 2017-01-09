use core::cell::Cell;
use kernel::hil::gpio;
use kernel::hil::spi;
use kernel::hil::radio;
use kernel::returncode::ReturnCode;
use kernel::common::take_cell::TakeCell;
use rf233_const::*;
use core::mem;

macro_rules! pinc_toggle {
    ($x:expr) => {
        unsafe {
            let toggle_reg: &mut u32 = mem::transmute(0x400E1000 + (2 * 0x200) + 0x5c);
            *toggle_reg = 1 << $x;
        }
    }
}

const D5: u32 = 28;
const D4: u32 = 29;
const C_BLUE: u32 = 29;
const C_PURPLE: u32 = 28;
const C_BLACK: u32 = 26;

#[allow(unused_variables, dead_code,non_camel_case_types)]
#[derive(Copy, Clone, PartialEq)]
enum InternalState {
    START,
    START_PART_READ,
    START_STATUS_READ,
    START_TURNING_OFF,
    START_CTRL1_SET,
    START_CCA_SET,
    START_PWR_SET,
    START_CTRL2_SET,
    START_IRQMASK_SET,
    START_XAH1_SET,
    START_XAH0_SET,
    START_PANID0_SET,
    START_PANID1_SET,
    START_IEEE0_SET,
    START_IEEE1_SET,
    START_IEEE2_SET,
    START_IEEE3_SET,
    START_IEEE4_SET,
    START_IEEE5_SET,
    START_IEEE6_SET,
    START_IEEE7_SET,
    START_SHORT0_SET,
    START_SHORT1_SET,
    START_CSMA_SEEDED,
    START_RPC_SET,

    ON_STATUS_READ,
    ON_PLL_WAITING,
    ON_PLL_SET,


    // Radio is in the RX_ON state, ready to receive packets
    READY,

    // States pertaining to packe transmission
    TX_STATUS_PRECHECK1,
    TX_WRITING_FRAME,
    TX_STATUS_PRECHECK2,
    TX_PLL_START,
    TX_PLL_WAIT,
    TX_ARET_ON,
    TX_TRANSMITTING,
    TX_DONE,
    TX_RETURN_TO_RX,

    // This state denotes we began a transmission, but
    // before we could transition to PLL_ON a packet began
    // to be received. When we handle the initial RX interrupt,
    // we'll transition to the correct state. We can't return to READY
    // because we need to block other operations.
    TX_PENDING,

    // Intermediate states when setting the short address
    // and PAN ID.
    CONFIG_SHORT0_SET,
    CONFIG_PAN0_SET,

    // This is a short-lived state for when software has detected
    // the chip is receiving a packet (by internal state) but has
    // not received the interrupt yet
    RX,
    RX_READING, // Starting to read a packet out of the radio
    RX_LEN_READ, // We've read the length of the frame
    RX_READ,    // Reading the packet out of the radio

    UNKNOWN,
}


pub struct RF233 <'a, S: spi::SpiMasterDevice + 'a> {
    spi: &'a S,
    radio_on: Cell<bool>,
    transmitting: Cell<bool>,
    receiving: Cell<bool>,
    spi_busy: Cell<bool>,
    interrupt_handling: Cell<bool>,
    interrupt_pending:  Cell<bool>,
    reset_pin: &'a gpio::Pin,
    sleep_pin: &'a gpio::Pin,
    irq_pin:   &'a gpio::Pin,
    irq_ctl:   &'a gpio::PinCtl,
    state: Cell<InternalState>,
    tx_buf: TakeCell<&'static mut [u8]>,
    tx_len: Cell<u8>,
    tx_client: Cell<Option<&'static radio::TxClient>>,
    rx_client: Cell<Option<&'static radio::RxClient>>,
    addr: Cell<u16>,
    pan: Cell<u16>,
    seq: Cell<u8>,
}

// 129 bytes because the max frame length is 128 and we need a byte for
// the SPI command/status code
static mut read_buf: [u8; 129] =  [0x0; 129];
static mut write_buf: [u8; 129] = [0x0; 129];

impl <'a, S: spi::SpiMasterDevice + 'a> spi::SpiMasterClient for RF233 <'a, S> {

    fn read_write_done(&self,
                       _write: &'static mut [u8],
                       _read: Option<&'static mut [u8]>,
                       _len: usize) {
        self.spi_busy.set(false);
        let handling = self.interrupt_handling.get();
        pinc_toggle!(C_BLUE);
        // This first case is when an interrupt fired during an SPI operation:
        // we wait for the SPI operation to complete then handle the
        // interrupt by reading the IRQ_STATUS register over the SPI.
        // Since itself is an SPI operation, return.
        if self.interrupt_pending.get() == true {
            self.interrupt_pending.set(false);
            self.handle_interrupt();
            return;
        }
        // This second case is when the SPI operation is reading the
        // IRQ_STATUS register from handling an interrupt. Note that
        // we're done handling the interrupt and continue with the
        // state machine. This is an else because handle_interrupt
        // sets interrupt_handling to true.
        if handling {
            self.interrupt_handling.set(false);
            let state = self.state.get();
            let interrupt = unsafe {
                read_buf[1]
            };
            if state == InternalState::ON_PLL_WAITING {
                if  (interrupt & IRQ_0_PLL_LOCK) == IRQ_0_PLL_LOCK {
                    self.state.set(InternalState::ON_PLL_SET);
                }
            }
            else if (state == InternalState::TX_TRANSMITTING &&
                     interrupt & IRQ_3_TRX_END == IRQ_3_TRX_END) {
                self.state.set(InternalState::TX_DONE);
            }
            if (interrupt & IRQ_2_RX_START == IRQ_2_RX_START) {
                // Start of frame
                pinc_toggle!(C_BLACK);
                self.receiving.set(true);
                self.state.set(InternalState::RX);
            }

            if (self.receiving.get() &&
                interrupt & IRQ_3_TRX_END == IRQ_3_TRX_END) {
                self.receiving.set(false);
                self.state.set(InternalState::RX_READING);
            }

        }

        match self.state.get() {
            // Default on state; wait for transmit() call or receive
            // interrupt
            InternalState::READY => { }

            // Starting state, begin start sequence.
            InternalState::START => {
                self.state_transition_read(RF233Register::IRQ_STATUS,
                                            InternalState::START_PART_READ);
            }
            InternalState::START_PART_READ => {
                self.state_transition_read(RF233Register::TRX_STATUS,
                                           InternalState::START_STATUS_READ);
            }
            InternalState::START_STATUS_READ => {
                unsafe {
                    let val = read_buf[0];
                    if val == ExternalState::ON as u8{
                        self.state_transition_write(RF233Register::TRX_STATE,
                                                    RF233TrxCmd::OFF as u8,
                                                    InternalState::START_TURNING_OFF);
                    } else {
                        // enable IRQ input
                        // clear IRQ
                        // enable IRQ interrrupt
                        self.state_transition_write(RF233Register::TRX_CTRL_1,
                                                    TRX_CTRL_1,
                                                    InternalState::START_CTRL1_SET);
                    }
                }
            }
            InternalState::START_TURNING_OFF => {
                self.irq_pin.make_input();
                self.irq_pin.clear();
                self.irq_ctl.set_input_mode(gpio::InputMode::PullNone);
                self.irq_pin.enable_interrupt(0, gpio::InterruptMode::RisingEdge);

                self.state_transition_write(RF233Register::TRX_CTRL_1,
                                            TRX_CTRL_1,
                                            InternalState::START_CTRL1_SET);
            }
            InternalState::START_CTRL1_SET => {
                self.state_transition_write(RF233Register::PHY_CC_CCA,
                                            PHY_CC_CCA,
                                            InternalState::START_CCA_SET);
            }
            InternalState::START_CCA_SET => {
                self.state_transition_write(RF233Register::PHY_TX_PWR,
                                            PHY_TX_PWR,
                                            InternalState::START_PWR_SET);
            }
            InternalState::START_PWR_SET => {
                self.state_transition_write(RF233Register::TRX_CTRL_2,
                                            TRX_CTRL_2,
                                            InternalState::START_CTRL2_SET)
            }
            InternalState::START_CTRL2_SET => {
                self.state_transition_write(RF233Register::IRQ_MASK,
                                            IRQ_MASK,
                                            InternalState::START_IRQMASK_SET);
            }

            InternalState::START_IRQMASK_SET => {
                self.state_transition_write(RF233Register::XAH_CTRL_1,
                                            XAH_CTRL_1,
                                            InternalState::START_XAH1_SET);
            }

            InternalState::START_XAH1_SET => {
                // This encapsulates the frame retry and CSMA retry
                // settings in the RF233 C code
                self.state_transition_write(RF233Register::XAH_CTRL_0,
                                            XAH_CTRL_0,
                                            InternalState::START_XAH0_SET);
            }
            InternalState::START_XAH0_SET => {
                self.state_transition_write(RF233Register::PAN_ID_0,
                                            (self.pan.get() >> 8) as u8,
                                            InternalState::START_PANID0_SET);
            }
            InternalState::START_PANID0_SET => {
                self.state_transition_write(RF233Register::PAN_ID_1,
                                            (self.pan.get() & 0xff) as u8,
                                            InternalState::START_PANID1_SET);
            }
            InternalState::START_PANID1_SET => {
                self.state_transition_write(RF233Register::IEEE_ADDR_0,
                                            IEEE_ADDR_0,
                                            InternalState::START_IEEE0_SET);
            }
            InternalState::START_IEEE0_SET => {
                self.state_transition_write(RF233Register::IEEE_ADDR_1,
                                            IEEE_ADDR_1,
                                            InternalState::START_IEEE1_SET);
            }
            InternalState::START_IEEE1_SET => {
                self.state_transition_write(RF233Register::IEEE_ADDR_2,
                                            IEEE_ADDR_2,
                                            InternalState::START_IEEE2_SET);
            }
            InternalState::START_IEEE2_SET => {
                self.state_transition_write(RF233Register::IEEE_ADDR_3,
                                            IEEE_ADDR_3,
                                            InternalState::START_IEEE3_SET);
            }
            InternalState::START_IEEE3_SET => {
                self.state_transition_write(RF233Register::IEEE_ADDR_4,
                                            IEEE_ADDR_4,
                                            InternalState::START_IEEE4_SET);
            }
            InternalState::START_IEEE4_SET => {
                self.state_transition_write(RF233Register::IEEE_ADDR_5,
                                            IEEE_ADDR_5,
                                            InternalState::START_IEEE5_SET);
            }
            InternalState::START_IEEE5_SET => {
                self.state_transition_write(RF233Register::IEEE_ADDR_6,
                                            IEEE_ADDR_6,
                                            InternalState::START_IEEE6_SET);
            }
            InternalState::START_IEEE6_SET => {
                self.state_transition_write(RF233Register::IEEE_ADDR_7,
                                            IEEE_ADDR_7,
                                            InternalState::START_IEEE7_SET);
            }
            InternalState::START_IEEE7_SET => {
                self.state_transition_write(RF233Register::SHORT_ADDR_0,
                                            (self.addr.get() >> 8) as u8,
                                            InternalState::START_SHORT0_SET);
            }
            InternalState::START_SHORT0_SET => {
                self.state_transition_write(RF233Register::SHORT_ADDR_1,
                                            (self.addr.get() & 0xff) as u8,
                                            InternalState::START_SHORT1_SET);
            }
            InternalState::START_SHORT1_SET => {
                self.state_transition_write(RF233Register::CSMA_SEED_0,
                                            SHORT_ADDR_0 + SHORT_ADDR_1,
                                            InternalState::START_CSMA_SEEDED);
            }
            InternalState::START_CSMA_SEEDED => {
                self.state_transition_write(RF233Register::TRX_RPC,
                                            TRX_RPC,
                                            InternalState::START_RPC_SET);
            }
            InternalState::START_RPC_SET => {
                // If asleep, turn on
                self.state_transition_read(RF233Register::TRX_STATUS,
                                           InternalState::ON_STATUS_READ);
            }
            InternalState::ON_STATUS_READ => {
                unsafe {
                    let val = read_buf[1];
                    self.state_transition_write(RF233Register::TRX_STATE,
                                                RF233TrxCmd::PLL_ON as u8,
                                                InternalState::ON_PLL_WAITING);
                }
            }
            InternalState::ON_PLL_WAITING => {
                // Waiting for the PLL interrupt, do nothing
            }

            // Final startup state, transition to READY and turn radio on.
            InternalState::ON_PLL_SET => {
                // We've completed the SPI operation to read the
                // IRQ_STATUS register, triggered by an interrupt
                // denoting moving to the PLL_ON state, so move
                // to RX_ON (see Sec 7, pg 36 of RF233 datasheet
                self.state_transition_write(RF233Register::TRX_STATE,
                                            RF233TrxCmd::RX_ON as u8,
                                            InternalState::READY);
            }
            InternalState::TX_STATUS_PRECHECK1 => {
                unsafe {
                    let status = read_buf[0] & 0x1f;
                    if (status == ExternalState::BUSY_RX_AACK as u8 ||
                        status == ExternalState::BUSY_TX_ARET as u8 ||
                        status == ExternalState::BUSY_RX as u8) {
                        self.state.set(InternalState::TX_PENDING);
                    } else {
                        self.state.set(InternalState::TX_WRITING_FRAME);
                        self.tx_buf.map(|wbuf| {
                            self.frame_write(wbuf, self.tx_len.get());
                        });
                    }
                }
            }
            InternalState::TX_WRITING_FRAME => {
                self.state_transition_read(RF233Register::TRX_STATUS,
                                           InternalState::TX_STATUS_PRECHECK2);
            }
            InternalState::TX_STATUS_PRECHECK2 => {
                unsafe {
                    let status = read_buf[0] & 0x1f;
                    if (status == ExternalState::BUSY_RX_AACK as u8 ||
                        status == ExternalState::BUSY_TX_ARET as u8 ||
                        status == ExternalState::BUSY_RX as u8) {
                        self.receiving.set(true);
                        self.state.set(InternalState::RX);
                    } else {
                        self.state_transition_write(RF233Register::TRX_STATE,
                                                    RF233TrxCmd:: PLL_ON as u8,
                                                    InternalState::TX_PLL_START);
                    }
                }
            }
            InternalState::TX_PLL_START => {
                self.state_transition_read(RF233Register::TRX_STATUS,
                                           InternalState::TX_PLL_WAIT);
            }
            InternalState::TX_PLL_WAIT => {
                self.transmitting.set(true);
                unsafe {
                    let status = read_buf[0] & 0x1f;
                    if status == ExternalState::STATE_TRANSITION_IN_PROGRESS as u8 {
                        self.state_transition_read(RF233Register::TRX_STATUS,
                                                   InternalState::TX_PLL_WAIT);
                    } else if status != ExternalState::PLL_ON as u8{
                        self.state_transition_write(RF233Register::TRX_STATE,
                                                    RF233TrxCmd::PLL_ON as u8,
                                                    InternalState::TX_PLL_WAIT);

                    } else {
                        self.state_transition_write(RF233Register::TRX_STATE,
                                                    RF233TrxCmd::TX_ARET_ON as u8,
                                                    InternalState::TX_ARET_ON);
                    }
                }
            }
            InternalState::TX_ARET_ON => {
                self.state_transition_write(RF233Register::TRX_STATE,
                                            RF233TrxCmd::TX_START as u8,
                                            InternalState::TX_TRANSMITTING);
            }
            InternalState::TX_TRANSMITTING => {
                // Do nothing, wait for TRX_END interrupt denoting transmission
                // completed. The code at the top of this SPI handler for
                // interrupt handling will transition to the TX_DONE state.
            }
            InternalState::TX_DONE => {
                self.state_transition_write(RF233Register::TRX_STATE,
                                            RF233TrxCmd::RX_ON as u8,
                                            InternalState::TX_RETURN_TO_RX);
            }
            InternalState::TX_RETURN_TO_RX => {
                unsafe {
                    let state = read_buf[0];
                    if state == ExternalState::RX_ON as u8 {
                        self.transmitting.set(false);
                        let buf = self.tx_buf.take();
                        self.state_transition_read(RF233Register::TRX_STATUS,
                                                   InternalState::READY);

                        self.tx_client.get().map(|c| {
                            c.send_done(buf.unwrap(), ReturnCode::SUCCESS);
                        });
                    }
                    else {
                        self.register_read(RF233Register::TRX_STATUS);
                    }
                }
            }

            // This state occurs when, in the midst of starting a
            // transmission, we discovered that the radio had moved into
            // a receive state. Since this will trigger interrupts,
            // we enter this dead state and just wait for the interrupt
            // handlers.
            InternalState::TX_PENDING => {}

            // No operations in the RX state, an SFD interrupt should
            // take us out of it.
            InternalState::RX => {}

            // Read the length out
            InternalState::RX_READING => {
                self.state.set(InternalState::RX_LEN_READ);
                unsafe {
                    self.frame_read(&mut read_buf, 1);
                }
            }

            InternalState::RX_LEN_READ => {
                self.state.set(InternalState::RX_READ);
                unsafe {
                    // Because the first byte of a frame read is
                    // the status of the chip, the first byte of the
                    // packet, the length field, is at index 1
                    self.frame_read(&mut read_buf, read_buf[1]);
                }
            }

            InternalState::RX_READ => {
                unsafe {
                    // Because the first byte of a frame read is
                    // the status of the chip, the first byte of the
                    // packet, the length field, is at index 1
                    let buf_len = read_buf[1];
                    for i in 0..buf_len as usize {
                        // Shift packet left by one
                        read_buf[i] = read_buf[i + 1];
                    }
                }
                self.receiving.set(false);
                // Just read a packet: if a transmission is pending,
                // start the transmission state machine
                if self.transmitting.get() {
                    self.state_transition_read(RF233Register::TRX_STATUS,
                                               InternalState::TX_STATUS_PRECHECK1);
                } else {
                    self.state_transition_read(RF233Register::TRX_STATUS,
                                               InternalState::READY);
                }
                self.rx_client.get().map(|client| {
                    unsafe {
                        client.receive(&read_buf, read_buf[0], ReturnCode::SUCCESS);
                    }
                });
            }

            InternalState::CONFIG_SHORT0_SET => {
                self.state_transition_write(RF233Register::SHORT_ADDR_1,
                                            (self.addr.get() >> 8) as u8,
                                            InternalState::READY);
            }
            InternalState::CONFIG_PAN0_SET => {
                self.state_transition_write(RF233Register::PAN_ID_1,
                                            (self.pan.get() >> 8) as u8,
                                            InternalState::READY);
            }
            InternalState::UNKNOWN => {}
        }
    }
}

impl<'a, S: spi::SpiMasterDevice + 'a> gpio::Client for  RF233 <'a, S> {
    fn fired(&self, identifier: usize) {
        self.handle_interrupt();
    }
}

impl<'a, S: spi::SpiMasterDevice + 'a> RF233 <'a, S> {
    pub fn new(spi: &'a S,
               reset: &'a gpio::Pin,
               sleep: &'a gpio::Pin,
               irq: &'a gpio::Pin,
               ctl: &'a gpio::PinCtl) -> RF233<'a, S> {
        RF233 {
            spi: spi,
            reset_pin: reset,
            sleep_pin: sleep,
            irq_pin: irq,
            irq_ctl: ctl,
            radio_on: Cell::new(false),
            transmitting: Cell::new(false),
            receiving: Cell::new(false),
            spi_busy: Cell::new(false),
            state: Cell::new(InternalState::START),
            interrupt_handling: Cell::new(false),
            interrupt_pending: Cell::new(false),
            tx_buf: TakeCell::empty(),
            tx_len: Cell::new(0),
            tx_client: Cell::new(None),
            rx_client: Cell::new(None),
            addr: Cell::new(0),
            pan: Cell::new(0),
            seq: Cell::new(0),
        }
    }

    fn handle_interrupt(&self) {
        // Because the first thing we do on handling an interrupt is
        // read the IRQ status, we defer handling the state transition
        // to the SPI handler
        pinc_toggle!(C_PURPLE);
        if self.spi_busy.get() == false {
            self.interrupt_handling.set(true);
            self.register_read(RF233Register::IRQ_STATUS);
        } else {
            self.interrupt_pending.set(true);
        }
    }

#[allow(dead_code)]
    fn register_write(&self,
                      reg: RF233Register,
                      val: u8) -> ReturnCode {

        if self.spi_busy.get() {return ReturnCode::EBUSY;}
        unsafe {
            write_buf[0] = (reg as u8) | RF233BusCommand::REGISTER_WRITE as u8;
            write_buf[1] = val;
            self.spi.read_write_bytes(&mut write_buf, Some(& mut read_buf), 2);
            self.spi_busy.set(true);
        }
        ReturnCode::SUCCESS
    }

    fn register_read(&self,
                     reg: RF233Register) -> ReturnCode {

        if self.spi_busy.get() {return ReturnCode::EBUSY;}
        unsafe {
            write_buf[0] = (reg as u8) | RF233BusCommand::REGISTER_READ as u8;
            write_buf[1] = 0;
            self.spi.read_write_bytes(&mut write_buf, Some(&mut read_buf), 2);
            self.spi_busy.set(true);
        }
        ReturnCode::SUCCESS
    }

    fn frame_write(&self,
                   buf: &mut [u8],
                   buf_len: u8) -> ReturnCode {
        if self.spi_busy.get() {return ReturnCode::EBUSY;}
        let write_len = (buf_len + 2) as usize;
        unsafe {
            write_buf[0] = RF233BusCommand::FRAME_WRITE as u8;
            {
                let mut slice = &mut write_buf[1..129];
                for (dst, src) in slice.iter_mut().zip(buf) {
                    *dst = *src;
                }
            }
            self.spi.read_write_bytes(&mut write_buf, Some(&mut read_buf), write_len);
            self.spi_busy.set(true);
        }
        ReturnCode::SUCCESS
    }

    fn frame_read(&self,
                  buf: &mut [u8],
                  buf_len: u8) -> ReturnCode {
        if self.spi_busy.get() {return ReturnCode::EBUSY;}
        let mut op_len: usize = buf_len as usize + 1; // Add one for the frame command
        unsafe {
            write_buf[0] = RF233BusCommand::FRAME_READ as u8;
            for i in 1..buf_len as usize {
                write_buf[i] = 0x00; // clear write buf for easier debugging
            }
            self.spi.read_write_bytes(&mut write_buf, Some(&mut read_buf), op_len);
            self.spi_busy.set(true);
        }
        ReturnCode::SUCCESS
    }


    fn state_transition_write(&self,
                              reg: RF233Register,
                              val: u8,
                              state: InternalState) {
        self.state.set(state);
        self.register_write(reg, val);
    }

    fn state_transition_read(&self,
                             reg: RF233Register,
                             state: InternalState) {
        self.state.set(state);
        self.register_read(reg);
    }

    /// Generate the 802.15.4 header and set up the radio's state to
    /// be able to send the packet (store reference, etc.).
    fn prepare_packet(&self, buf: &'static mut [u8], len: u8, dest: u16) {

        buf[0] = len + 2 - 1; // plus 2 for CRC, - 1 for length byte  1/6/17 PAL
        buf[1] = 0x61;
        buf[2] = 0xAA;
        buf[3] = self.seq.get();
        self.seq.set(self.seq.get() + 1);
        buf[4] = (self.pan.get() & 0xFF) as u8;
        buf[5] = (self.pan.get() >> 8) as u8;
        buf[6] = (dest & 0xff) as u8;
        buf[7] = (dest >> 8) as u8;
        buf[8] = (self.addr.get() & 0xFF) as u8;
        buf[9] = (self.addr.get() >> 8) as u8;

        self.tx_buf.replace(buf);
        self.tx_len.set(len);
    }
}

impl<'a, S: spi::SpiMasterDevice + 'a> radio::Radio for RF233 <'a, S> {
    fn initialize(&self) -> ReturnCode {
        self.spi.configure(spi::ClockPolarity::IdleLow,
                           spi::ClockPhase::SampleLeading,
                           100000);
        self.reset()
    }

    fn reset(&self) -> ReturnCode {
        self.reset_pin.make_output();
        self.sleep_pin.make_output();
        for i in 0..10000 {
            self.reset_pin.clear();
        }
        self.reset_pin.set();
        self.sleep_pin.clear();
        self.transmitting.set(false);
        self.radio_on.set(true);
        ReturnCode::SUCCESS
    }

#[allow(dead_code)]
    fn start(&self) -> ReturnCode {
        if self.state.get() != InternalState::START {
            return ReturnCode::FAIL;
        }
        self.register_read(RF233Register::PART_NUM);
        ReturnCode::SUCCESS
    }

    fn stop(&self) -> ReturnCode {
        ReturnCode::FAIL
    }

    fn set_transmit_client(&self, client: &'static radio::TxClient) {
        self.tx_client.set(Some(client));
    }
    fn set_receive_client(&self, client: &'static radio::RxClient) {
        self.rx_client.set(Some(client));
    }

    fn set_address(&self, addr: u16) -> ReturnCode {
        let state = self.state.get();
        // The start state will push addr into hardware on initialization;
        // the ready state needs to do so immediately.
        if (state == InternalState::READY ||
            state == InternalState::START) {
            self.addr.set(addr);
            if (state == InternalState::READY) {
                self.state_transition_write(RF233Register::SHORT_ADDR_0,
                                            (self.addr.get() & 0xff) as u8,
                                            InternalState::CONFIG_SHORT0_SET);
            }
            ReturnCode::SUCCESS
        }
        else {
            ReturnCode::EBUSY
        }
    }

    fn set_pan(&self, addr: u16) -> ReturnCode {
        let state = self.state.get();
        // The start state will push addr into hardware on initialization;
        // the ready state needs to do so immediately.
        if (state == InternalState::READY ||
            state == InternalState::START) {
            self.pan.set(addr);
            if (state == InternalState::READY) {
                self.state_transition_write(RF233Register::PAN_ID_0,
                                            (self.pan.get() & 0xff) as u8,
                                            InternalState::CONFIG_PAN0_SET);
            }
            ReturnCode::SUCCESS
        }
        else {
            ReturnCode::EBUSY
        }
    }

    fn payload_offset(&self) -> u8 {
        radio::HEADER_SIZE
    }
    fn header_size(&self) -> u8 {
        radio::HEADER_SIZE
    }

    fn transmit(&self,
                dest: u16,
                payload: &'static mut [u8],
                len: u8) -> ReturnCode {
        let state = self.state.get();
        if state == InternalState::START {
            return ReturnCode::EOFF;
        }  else if (self.tx_buf.is_some() ||
                    self.transmitting.get()) {
            return ReturnCode::EBUSY;
        }

        self.prepare_packet(payload, len, dest);
        self.transmitting.set(true);
        if !self.receiving.get() {
            self.state_transition_read(RF233Register::TRX_STATUS,
                                       InternalState::TX_STATUS_PRECHECK1);
        }
        return ReturnCode::SUCCESS;
    }

}
