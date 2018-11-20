; ModuleID = 'tests/integration/enums'
source_filename = "tests/integration/enums"

%Str = type { i8*, i64 }
%Sat = type { i8 }

@_strlit.5 = private unnamed_addr constant [8 x i8] c"Weekend\00", align 1

; Function Attrs: nounwind
define i32 @main(i32, i8** nocapture readnone) local_unnamed_addr #0 {
entry:
  tail call void @print(%Str { i8* getelementptr inbounds ([8 x i8], [8 x i8]* @_strlit.5, i32 0, i32 0), i64 7 })
  ret i32 0
}

; Function Attrs: norecurse nounwind readnone
define %Sat @getDay() local_unnamed_addr #1 {
entry:
  ret i64 93906207806928
}

; Function Attrs: nounwind
define void @print(%Str) local_unnamed_addr #0 {
entry:
  %1 = extractvalue %Str %0, 0
  %2 = tail call i32 @puts(i8* %1)
  ret void
}

; Function Attrs: nounwind
declare i32 @puts(i8* nocapture readonly) local_unnamed_addr #0

attributes #0 = { nounwind }
attributes #1 = { norecurse nounwind readnone }
