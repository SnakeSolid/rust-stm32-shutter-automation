#![no_main]
#![no_std]

mod command;
mod controller;
mod string;

use crate::command::Command;
use crate::controller::ControlChannel;
use crate::controller::Controller;
use crate::string::String;
use core::cell::RefCell;
use core::panic::PanicInfo;
use cortex_m::asm::wfi;
use cortex_m::interrupt::free;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use nb::block;
use stm32f1xx_hal::afio::AfioExt;
use stm32f1xx_hal::device::TIM2;
use stm32f1xx_hal::dma::Transfer;
use stm32f1xx_hal::flash::FlashExt;
use stm32f1xx_hal::gpio::GpioExt;
use stm32f1xx_hal::pac;
use stm32f1xx_hal::pac::interrupt;
use stm32f1xx_hal::pac::Interrupt;
use stm32f1xx_hal::pac::USART1;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::rcc::RccExt;
use stm32f1xx_hal::serial::Config;
use stm32f1xx_hal::serial::Rx;
use stm32f1xx_hal::serial::Serial;
use stm32f1xx_hal::time::U32Ext;
use stm32f1xx_hal::timer::CounterHz;
use stm32f1xx_hal::timer::Event as TimerEvent;
use stm32f1xx_hal::timer::TimerExt;

static USART_RX: Mutex<RefCell<Option<Rx<USART1>>>> = Mutex::new(RefCell::new(None));

static TIMER: Mutex<RefCell<Option<CounterHz<TIM2>>>> = Mutex::new(RefCell::new(None));

const MAX_LINE_LENGTH: usize = 32;

static BUFFER: Mutex<RefCell<String<MAX_LINE_LENGTH>>> = Mutex::new(RefCell::new(String::new()));

static LINE: Mutex<RefCell<Option<String<MAX_LINE_LENGTH>>>> = Mutex::new(RefCell::new(None));

static MILLIS: Mutex<RefCell<u32>> = Mutex::new(RefCell::new(0));

macro_rules! send_buffer {
    ($transfer:ident, $buffer:expr) => {{
        let tx = $transfer.wait().1;

        tx.write($buffer)
    }};
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let _cp = cortex_m::Peripherals::take().unwrap();
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();
    let clocks = rcc
        .cfgr
        .sysclk(4.MHz())
        .adcclk(4.MHz())
        .freeze(&mut flash.acr);
    let channels = dp.DMA1.split();
    let mut afio = dp.AFIO.constrain();
    let mut gpiob = dp.GPIOB.split();
    let mut gpioc = dp.GPIOC.split();

    let mut channel1_up = gpiob.pb12.into_push_pull_output(&mut gpiob.crh);
    let mut channel1_down = gpiob.pb13.into_push_pull_output(&mut gpiob.crh);
    let mut channel2_up = gpiob.pb14.into_push_pull_output(&mut gpiob.crh);
    let mut channel2_down = gpiob.pb15.into_push_pull_output(&mut gpiob.crh);
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

    let tx = gpiob.pb6.into_alternate_push_pull(&mut gpiob.crl);
    let rx = gpiob.pb7;
    let (usart_tx, mut usart_rx) = Serial::usart1(
        dp.USART1,
        (tx, rx),
        &mut afio.mapr,
        Config::default().baudrate(9_600.bps()),
        clocks,
    )
    .split();
    let usart_tx = usart_tx.with_dma(channels.4);
    usart_rx.listen();

    let mut timer = dp.TIM2.counter_hz(&clocks);
    timer.start(1_000.Hz()).unwrap();
    timer.listen(TimerEvent::Update);

    free(|cs| {
        *USART_RX.borrow(cs).borrow_mut() = Some(usart_rx);
        *TIMER.borrow(cs).borrow_mut() = Some(timer);
    });

    unsafe {
        pac::NVIC::unmask(Interrupt::USART1);
        pac::NVIC::unmask(Interrupt::TIM2);
    }

    let mut controller = Controller::new([
        ControlChannel::new(&mut channel1_up, &mut channel1_down),
        ControlChannel::new(&mut channel2_up, &mut channel2_down),
    ]);
    controller.stop_all();

    let mut transfer: Transfer<_, &[u8], _> = usart_tx.write(b"Ready\n");

    loop {
        if let Some(line) = free(|cs| LINE.borrow(cs).borrow_mut().take()) {
            match Command::parse(line.as_ref()) {
                Ok(Command::Stop { index: None }) => {
                    controller.stop_all();

                    transfer = send_buffer!(transfer, b"Ok\n");
                }
                Ok(Command::Stop { index: Some(index) }) => {
                    controller.stop(index as usize);

                    transfer = send_buffer!(transfer, b"Ok\n");
                }
                Ok(Command::Up { index }) => {
                    controller.up(index as usize);

                    transfer = send_buffer!(transfer, b"Ok\n");
                }
                Ok(Command::Down { index }) => {
                    controller.down(index as usize);

                    transfer = send_buffer!(transfer, b"Ok\n");
                }
                Ok(Command::Limit {
                    index,
                    up_limit,
                    down_limit,
                }) => {
                    controller.limit(index as usize, up_limit, down_limit);

                    transfer = send_buffer!(transfer, b"Ok\n");
                }
                Ok(Command::Help) => {
                    transfer = send_buffer!(transfer, include_bytes!("help.txt"));
                }
                Err(_) => {
                    transfer = send_buffer!(transfer, b"Err: Unknown command\n");
                }
            }
        }

        let delta = get_elapsed_time();
        controller.update(delta);

        match controller.is_active() {
            true => led.set_low(),
            false => led.set_high(),
        }

        wfi();
    }
}

fn get_elapsed_time() -> u32 {
    free(|cs| {
        let mut millis = MILLIS.borrow(cs).borrow_mut();
        let result = *millis;
        *millis = 0;

        result
    })
}

#[interrupt]
fn USART1() {
    free(|cs| {
        if let Some(usart_rx) = USART_RX.borrow(cs).borrow_mut().as_mut() {
            if usart_rx.is_rx_not_empty() {
                let mut line = LINE.borrow(cs).borrow_mut();
                let mut buffer = BUFFER.borrow(cs).borrow_mut();

                match block!(usart_rx.read()) {
                    Ok(b'\r') | Ok(b'\n') => {
                        line.replace(buffer.clone());
                        buffer.clear();
                    }
                    Ok(ch) if ch < b' ' => {}
                    Ok(ch) => buffer.push(ch),
                    _ => {}
                }
            }
        }
    });
}

#[interrupt]
fn TIM2() {
    free(|cs| {
        if let Some(timer) = TIMER.borrow(cs).borrow_mut().as_mut() {
            let _ = timer.wait();
        }

        let mut millis = MILLIS.borrow(cs).borrow_mut();
        *millis += 1;
    });
}

#[panic_handler]
unsafe fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
