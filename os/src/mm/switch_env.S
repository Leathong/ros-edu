    .equ XLENB, 8
    .equ PERMISSION_XRWV, 0b1111
    .macro LOAD_SP a1, a2
        ld \a1, \a2*XLENB(sp)
    .endm
    .macro STORE_SP a1, a2
        sd \a1, \a2*XLENB(sp)
    .endm
    
    .global _ptenv_switch
_ptenv_switch:
    mv s1, sp
    addi sp, sp, -2*XLENB
    
    li t0, 1 << 1
    csrrc s2, sstatus, t0
    csrr s3, satp

    # save a0
    STORE_SP a0, 0
    STORE_SP ra, 1
    # jump to init_pt_addr
    la t0, switch_to_pt
    jalr ra, 0(t0) 

    LOAD_SP a0, 0
    LOAD_SP ra, 1

    addi sp, sp, 2*XLENB
    ret
    .global _ptenv_restore
_ptenv_restore:
    csrw satp, s3
    csrw sstatus, s2
    mv sp, s1
    ret

switch_to_pt:
    # load page table
    la t0, identical_map_pt_addr
    ld t0, 0(t0)
    srli t0, t0, 12      
    li t1, (0x8 << 60)   
    or t0, t0, t1   
    csrw satp, t0
    ret

identical_map_pt_addr:
    .dword identical_map_pt