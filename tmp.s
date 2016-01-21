	.text
	.file	"tmp"
	.globl	main
	.align	16, 0x90
	.type	main,@function
main:                                   # @main
	.cfi_startproc
# BB#0:                                 # %entry
	pushq	%rax
.Ltmp0:
	.cfi_def_cfa_offset 16
	movl	$.L__unnamed_1, %edi
	callq	puts
	xorl	%eax, %eax
	popq	%rdx
	retq
.Lfunc_end0:
	.size	main, .Lfunc_end0-main
	.cfi_endproc

	.type	.L__unnamed_1,@object   # @0
	.section	.rodata.str1.1,"aMS",@progbits,1
.L__unnamed_1:
	.asciz	"Hello World!"
	.size	.L__unnamed_1, 13


	.section	".note.GNU-stack","",@progbits
