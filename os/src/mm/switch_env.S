    .equ XLENB, 8
    .equ PERMISSION_XRWV, 0b1111
    .macro LOAD_SP a1, a2
        ld \a1, \a2*XLENB(sp)
    .endm
    .macro STORE_SP a1, a2
        sd \a1, \a2*XLENB(sp)
    .endm

identical_map_pt_addr:
    .dword identical_map_pt

    .section .data.ptenv
    .align 12
ptenv_stack:
    .space 4096 * 4
ptenv_stack_top: