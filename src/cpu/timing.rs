extern "C" {
    fn start_clk(clk: *mut SysClk, cb: extern fn(*mut SysClk, u16));
}

#[repr(C)]
pub struct SysClk {
    pub div: u16,
    pub tma: u8,
    pub tima: u8,
}

#[no_mangle]
extern "C" fn update_div(clk: *mut SysClk, div: u16) {
    unsafe {
        (*clk).div = div;
    }
}

impl SysClk {
    pub fn start() -> Box<SysClk> {
        let mut clk = Box::new(SysClk { div: 0, tma: 0, tima: 0 });
        unsafe {
            start_clk(&mut *clk, update_div);
        }
        return clk;
    }
}