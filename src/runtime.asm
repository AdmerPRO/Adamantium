; Adamantium Windows x64 entry point and typed runtime calls.
bits 64
default rel
global main
extern SetConsoleOutputCP
extern ExitProcess
extern ad_evaluate
extern ad_print
extern ad_message
extern ad_object_new
extern ad_object_clone
extern ad_list_error

section .text
main:
    sub rsp, 40
    mov ecx, 65001
    call SetConsoleOutputCP
    call ad_fun_main
    xor ecx, ecx
    call ExitProcess

; Runtime helpers return a nonzero exit code on failure.
ad_exit_error:
    mov ecx, eax
    call ExitProcess
