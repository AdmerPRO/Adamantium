; Adamantium runtime: Windows x64, signed 64-bit integers, UTF-8 output.
bits 64
default rel
global main
extern GetStdHandle
extern SetConsoleOutputCP
extern WriteFile
extern ExitProcess

section .text
main:
    sub rsp, 40
    mov ecx, 65001
    call SetConsoleOutputCP
    call ad_fun_main
    xor ecx, ecx
    call ExitProcess

; RCX = bytes, RDX = length.
ad_write:
    push rbp
    mov rbp, rsp
    sub rsp, 64
    mov [rbp - 8], rcx
    mov [rbp - 16], rdx
    mov ecx, -11
    call GetStdHandle
    mov rcx, rax
    mov rdx, [rbp - 8]
    mov r8, [rbp - 16]
    lea r9, [rbp - 24]
    mov qword [rsp + 32], 0
    call WriteFile
    test eax, eax
    jz .error
    mov eax, [rbp - 24]
    cmp rax, [rbp - 16]
    jne .error
    mov rsp, rbp
    pop rbp
    ret
.error:
    mov ecx, 1
    call ExitProcess

; RCX = integer. Unsigned magnitude conversion also handles INT64_MIN.
ad_print_integer:
    sub rsp, 72
    mov rax, rcx
    mov r11, rcx
    lea r9, [rsp + 72]
    mov r10, r9
    test rax, rax
    jns .digits
    neg rax
.digits:
    xor edx, edx
    mov ecx, 10
    div rcx
    add dl, '0'
    dec r9
    mov [r9], dl
    test rax, rax
    jnz .digits
    test r11, r11
    jns .write
    dec r9
    mov byte [r9], '-'
.write:
    mov rcx, r9
    mov rdx, r10
    sub rdx, r9
    call ad_write
    add rsp, 72
    ret

; Generated failure branches arrive with RSP aligned to 16 bytes.
ad_runtime_error:
    sub rsp, 64
    mov ecx, -12
    call GetStdHandle
    mov rcx, rax
    lea rdx, [rel ad_error_message]
    mov r8d, ad_error_length
    lea r9, [rsp + 48]
    mov qword [rsp + 32], 0
    call WriteFile
    mov ecx, 2
    call ExitProcess

section .rdata
ad_error_message: db "Adamantium runtime error: arithmetic overflow, division by zero or invalid clamp range.", 13, 10
ad_error_length equ $ - ad_error_message
section .text
