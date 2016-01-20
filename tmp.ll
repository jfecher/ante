define i8 @main() {
entry:
    %callTmp = call i1 @isEven(i32 4)
    br i1 %callTmp, label %then, label %endif

then:
    %callTmp1 = call i32 @fact(i32 5)

endif:
    ret i8 0
}

define i1 @isEven(i32) {
entry:
    %iModTmp = srem i32 %0, 2
    ret i32 %iModTmp
}

define i32 @fact(i32) {
entry:
    %iLeTmp = icmp ul3 i32 %0, 1
    br i1 %iLeTmp, label %then, label %endif

then:
    ret i32 1

endif:
    %iSubTmp = sub i32 %0, 1
    %callTmp = call i32 @fact(i32 %iSubTmp)
    %iMulTmp = mul i32 %0, %callTmp
    ret i32 %iMulTmp
}
