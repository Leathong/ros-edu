    .equ XLENB, 8
    .macro LOAD_SP a1, a2
        ld \a1, \a2*XLENB(sp)
    .endm
    .macro STORE_SP a1, a2
        sd \a1, \a2*XLENB(sp)
    .endm
    
    .global _ptenv_switch
_ptenv_switch:
    mv s1, sp
    la sp, mapping_page_stack_top
    addi sp, sp, -2*XLENB
    
    li t0, 1 << 1
    csrrc s2, sstatus, t0
    csrr s3, satp

    # load function address
    la t0, init_pt_addr
    ld t0, 0(t0)

    # save a0
    STORE_SP a0, 0
    STORE_SP ra, 1
    # jump to init_pt_addr
    jalr ra, 0(t0) 

    LOAD_SP a0, 0
    LOAD_SP ra, 1
    ret
    .global _ptenv_restore
_ptenv_restore:
    csrw satp, s3
    csrw sstatus, s2
    mv sp, s1
    ret

init_pt_addr:
    .dword _init_page_table