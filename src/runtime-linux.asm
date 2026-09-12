; Adamantium Linux x86-64 entry point and Windows-register compatibility wrappers.
bits 64
default rel
global main
extern exit
extern ad_evaluate
extern ad_print
extern ad_message
extern ad_object_new
extern ad_object_clone
extern ad_list_error
extern ad_optional_error
extern ad_parse_arguments
extern ad_try_begin
extern ad_try_end
extern ad_has_error
extern ad_is_trying
extern ad_package_call

section .text
main:
    sub rsp, 8
    mov rcx, rdi
    mov rdx, rsi
    call ad_cli_main
    xor eax, eax
    add rsp, 8
    ret

ad_exit_error:
    mov edi, eax
    call exit

ad_linux_exit:
    mov edi, ecx
    call exit

ad_linux_evaluate:
    mov rdi, rcx
    sub rsp, 8
    call ad_evaluate
    add rsp, 8
    ret

ad_linux_print:
    mov rdi, rcx
    mov esi, edx
    mov edx, r8d
    sub rsp, 8
    call ad_print
    add rsp, 8
    ret

ad_linux_message:
    mov rdi, rcx
    mov esi, edx
    mov edx, r8d
    sub rsp, 8
    call ad_message
    add rsp, 8
    ret

ad_linux_object_new:
    mov edi, ecx
    sub rsp, 8
    call ad_object_new
    add rsp, 8
    ret

ad_linux_object_clone:
    mov rdi, rcx
    mov esi, edx
    sub rsp, 8
    call ad_object_clone
    add rsp, 8
    ret

ad_linux_list_error:
    mov rdi, rcx
    mov rsi, rdx
    sub rsp, 8
    call ad_list_error
    add rsp, 8
    ret

ad_linux_optional_error:
    sub rsp, 8
    call ad_optional_error
    add rsp, 8
    ret

ad_linux_parse_arguments:
    mov rdi, rcx
    mov rsi, rdx
    mov rdx, r8
    mov rcx, r9
    mov r8, [rsp + 40]
    sub rsp, 8
    call ad_parse_arguments
    add rsp, 8
    ret

ad_linux_try_begin:
    sub rsp, 8
    call ad_try_begin
    add rsp, 8
    ret

ad_linux_try_end:
    mov rdi, rcx
    sub rsp, 8
    call ad_try_end
    add rsp, 8
    ret

ad_linux_has_error:
    sub rsp, 8
    call ad_has_error
    add rsp, 8
    ret

ad_linux_is_trying:
    sub rsp, 8
    call ad_is_trying
    add rsp, 8
    ret

ad_linux_package_call:
    mov rdi, rcx
    sub rsp, 8
    call ad_package_call
    add rsp, 8
    ret
