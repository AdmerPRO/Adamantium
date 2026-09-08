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
extern ad_optional_error
extern ad_parse_arguments

section .text
main:
    sub rsp, 56
    mov [rsp + 40], rcx
    mov [rsp + 48], rdx
    mov ecx, 65001
    call SetConsoleOutputCP
    mov rcx, [rsp + 40]
    mov rdx, [rsp + 48]
    call ad_cli_main
    xor ecx, ecx
    call ExitProcess

; Runtime helpers return a nonzero exit code on failure.
ad_exit_error:
    mov ecx, eax
    call ExitProcess
