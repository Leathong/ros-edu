//! Implementation of [`TrapFrame`]
use core::ops::{Deref, DerefMut};

#[repr(C)]
#[derive(Debug, Default)]
///trap context structure containing sstatus, sepc and registers
pub struct TrapFrame {
    /// General registers, 0 ~ 31
    pub general: GeneralRegs,
    /// Supervisor Status, 32
    pub sstatus: usize,
    /// Supervisor Exception Program Counter, 33
    pub sepc: usize,
}

/// General registers
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct GeneralRegs {
    pub zero: usize,
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct UserContext(TrapFrame);

impl Deref for UserContext {
    type Target = TrapFrame;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for UserContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[allow(unused)]
impl UserContext {
    /// Get number of syscall
    pub fn get_syscall_num(&self) -> usize {
        self.general.a7
    }

    /// Get return value of syscall
    pub fn get_syscall_ret(&self) -> usize {
        self.general.a0
    }

    /// Set return value of syscall
    pub fn set_syscall_ret(&mut self, ret: usize) {
        self.general.a0 = ret;
    }

    /// Get syscall args
    pub fn get_syscall_args(&self) -> [usize; 3] {
        [
            self.general.a0,
            self.general.a1,
            self.general.a2,
            // self.general.a3,
            // self.general.a4,
            // self.general.a5,
        ]
    }

    /// Set instruction pointer
    pub fn set_ip(&mut self, ip: usize) {
        self.sepc = ip;
    }

    /// Set stack pointer
    pub fn set_sp(&mut self, sp: usize) {
        self.0.set_sp(sp);
    }

    /// Get stack pointer
    pub fn get_sp(&self) -> usize {
        self.general.sp
    }

    /// Set tls pointer
    pub fn set_tls(&mut self, tls: usize) {
        self.general.gp = tls;
    }

    pub fn run(&self) {
        unsafe {
            run_user(self);
        }
    }
}

impl TrapFrame {
    /// Set stack pointer
    pub fn set_sp(&mut self, sp: usize) {
        self.general.sp = sp;
    }
}

unsafe extern "C" {
    fn run_user(ctx: &UserContext);
}
