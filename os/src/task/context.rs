#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub(crate) struct TaskContext {
    pub regs: CalleeRegs, // 0..12
    pub pc: usize,        // 13
    pub tls: usize,       // 14
}

/// Callee-saved registers.
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct CalleeRegs {
    /// sp
    pub sp: u64,
    /// s0
    pub s0: u64,
    /// s1
    pub s1: u64,
    /// s2
    pub s2: u64,
    /// s3
    pub s3: u64,
    /// s4
    pub s4: u64,
    /// s5
    pub s5: u64,
    /// s6
    pub s6: u64,
    /// s7
    pub s7: u64,
    /// s8
    pub s8: u64,
    /// s9
    pub s9: u64,
    /// s10
    pub s10: u64,
    /// s11
    pub s11: u64,
}

impl CalleeRegs {
    /// Creates new `CalleeRegs`
    pub const fn new() -> Self {
        CalleeRegs {
            sp: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
        }
    }
}

#[allow(unused)]
impl TaskContext {
    pub const fn default() -> Self {
        TaskContext {
            regs: CalleeRegs::new(),
            pc: 0,
            tls: 0,
        }
    }

    /// Sets thread-local storage pointer.
    pub fn set_tls_pointer(&mut self, tls: usize) {
        self.tls = tls;
    }

    /// Gets thread-local storage pointer.
    pub fn tls_pointer(&self) -> usize {
        self.tls
    }

    pub fn set_instruction_pointer(&mut self, ip: usize) {
        self.pc = ip;
    }

    pub fn instruction_pointer(&self) -> usize {
        self.pc
    }

    pub fn set_stack_pointer(&mut self, sp: usize) {
        self.regs.sp = sp as u64;
    }

    pub fn stack_pointer(&self) -> usize {
        self.regs.sp as usize
    }
}
