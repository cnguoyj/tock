Tock Chips
==========

The `/chips` folder contains the list of microcontrollers supported by Tock.
Each MCU folder contains the hardware peripheral drivers for that MCU.



HIL Support
-----------

<!--START OF HIL SUPPORT-->

| HIL                                     | apollo3 | arty_e21_chip | e310x | earlgrey | imxrt10xx | litex | litex_vexriscv | lowrisc | msp432 | nrf52832 | nrf52833 | nrf52840 | sam4l | stm32f303xc | stm32f412g | stm32f429zi | stm32f446re | stm32f4xx |
|-----------------------------------------|---------|---------------|-------|----------|-----------|-------|----------------|---------|--------|----------|----------|----------|-------|-------------|------------|-------------|-------------|-----------|
| adc::Adc                                |         |               |       |          |           |       |                |         | ✓      | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| adc::AdcHighSpeed                       |         |               |       |          |           |       |                |         | ✓      |          |          |          | ✓     | ✓           |            |             |             | ✓         |
| analog_comparator::AnalogComparator     |         |               |       |          |           |       |                |         |        | ✓        |          | ✓        | ✓     |             |            |             |             |           |
| ble_advertising::BleAdvertisementDriver | ✓       |               |       |          |           |       |                |         |        | ✓        |          | ✓        |       |             |            |             |             |           |
| ble_advertising::BleConfig              | ✓       |               |       |          |           |       |                |         |        | ✓        |          | ✓        |       |             |            |             |             |           |
| bus8080::Bus8080                        |         |               |       |          |           |       |                |         |        |          |          |          |       |             |            |             |             | ✓         |
| crc::CRC                                |         |               |       |          |           |       |                |         |        |          |          |          | ✓     |             |            |             |             |           |
| dac::DacChannel                         |         |               |       |          |           |       |                |         |        |          |          |          | ✓     |             |            |             |             |           |
| digest::Digest                          |         |               |       |          |           |       |                | ✓       |        |          |          |          |       |             |            |             |             |           |
| digest::HMACSha256                      |         |               |       |          |           |       |                | ✓       |        |          |          |          |       |             |            |             |             |           |
| eic::ExternalInterruptController        |         |               |       |          |           |       |                |         |        |          |          |          | ✓     |             |            |             |             |           |
| entropy::Entropy32                      |         |               |       |          |           |       |                |         |        | ✓        |          | ✓        | ✓     |             |            |             |             | ✓         |
| flash::Flash                            |         |               |       |          |           |       |                | ✓       |        | ✓        |          | ✓        | ✓     | ✓           |            |             |             |           |
| gpio::Input                             | ✓       |               | ✓     |          | ✓         |       |                | ✓       |        | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| gpio::Interrupt                         | ✓       |               | ✓     |          | ✓         |       |                | ✓       | ✓      | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| gpio::InterruptPin                      | ✓       |               | ✓     |          | ✓         |       |                | ✓       | ✓      | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| gpio::Output                            | ✓       |               | ✓     |          | ✓         |       |                | ✓       |        | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| gpio::Pin                               | ✓       |               | ✓     |          | ✓         |       |                | ✓       |        | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| i2c::I2CMaster                          | ✓       |               |       |          | ✓         |       |                | ✓       |        | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| i2c::I2CMasterSlave                     |         |               |       |          |           |       |                |         |        |          |          |          | ✓     |             |            |             |             |           |
| i2c::I2CSlave                           |         |               |       |          |           |       |                |         |        |          |          |          | ✓     |             |            |             |             |           |
| i2c::SMBusMaster                        | ✓       |               |       |          |           |       |                |         |        |          |          |          |       |             |            |             |             |           |
| led::Led                                |         |               |       |          |           | ✓     |                |         |        |          |          |          |       |             |            |             |             |           |
| mod::Controller                         |         |               |       |          |           |       |                |         |        |          |          |          | ✓     |             |            |             |             |           |
| pwm::Pwm                                |         |               |       |          |           |       |                |         |        | ✓        |          | ✓        |       |             |            |             |             |           |
| radio::Radio                            |         |               |       |          |           |       |                |         |        | ✓        |          | ✓        |       |             |            |             |             |           |
| radio::RadioConfig                      |         |               |       |          |           |       |                |         |        | ✓        |          | ✓        |       |             |            |             |             |           |
| radio::RadioData                        |         |               |       |          |           |       |                |         |        | ✓        |          | ✓        |       |             |            |             |             |           |
| sensors::TemperatureDriver              |         |               |       |          |           |       |                |         |        | ✓        |          | ✓        |       |             |            |             |             |           |
| spi::SpiMaster                          |         |               |       |          |           |       |                |         |        | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| spi::SpiSlave                           |         |               |       |          |           |       |                |         |        |          |          |          | ✓     |             |            |             |             |           |
| symmetric_encryption::AES128            |         |               |       | ✓        |           |       |                |         |        | ✓        |          | ✓        | ✓     |             |            |             |             |           |
| symmetric_encryption::AES128CBC         |         |               |       |          |           |       |                |         |        | ✓        |          | ✓        | ✓     |             |            |             |             |           |
| symmetric_encryption::AES128CCM         |         |               |       |          |           |       |                |         |        | ✓        |          | ✓        |       |             |            |             |             |           |
| symmetric_encryption::AES128Ctr         |         |               |       |          |           |       |                |         |        | ✓        |          | ✓        | ✓     |             |            |             |             |           |
| symmetric_encryption::AES128ECB         |         |               |       | ✓        |           |       |                |         |        |          |          |          |       |             |            |             |             |           |
| time::Alarm                             | ✓       |               |       | ✓        | ✓         |       |                |         | ✓      | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| time::Counter                           | ✓       |               |       | ✓        |           |       |                |         | ✓      | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| time::Frequency                         |         |               |       | ✓        | ✓         | ✓     |                |         | ✓      |          |          |          |       |             |            |             |             |           |
| time::Time                              | ✓       |               |       | ✓        | ✓         | ✓     |                |         | ✓      | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| time::Timer                             |         |               |       |          |           | ✓     |                |         |        |          |          |          |       |             |            |             |             |           |
| uart::Configure                         | ✓       |               | ✓     |          | ✓         | ✓     |                | ✓       | ✓      | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| uart::Receive                           | ✓       |               | ✓     |          | ✓         | ✓     |                | ✓       | ✓      | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| uart::ReceiveAdvanced                   |         |               |       |          |           |       |                |         |        |          |          |          | ✓     |             |            |             |             |           |
| uart::Transmit                          | ✓       |               | ✓     |          | ✓         | ✓     |                | ✓       | ✓      | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| uart::Uart                              | ✓       |               | ✓     |          | ✓         | ✓     |                | ✓       | ✓      | ✓        |          | ✓        | ✓     | ✓           |            |             |             | ✓         |
| uart::UartAdvanced                      |         |               |       |          |           |       |                |         |        |          |          |          | ✓     |             |            |             |             |           |
| uart::UartData                          | ✓       |               | ✓     |          | ✓         | ✓     |                | ✓       | ✓      | ✓        |          | ✓        |       | ✓           |            |             |             | ✓         |
| usb::UsbController                      |         |               |       |          |           |       |                | ✓       |        | ✓        |          | ✓        | ✓     |             |            |             |             |           |

<!--END OF HIL SUPPORT-->


