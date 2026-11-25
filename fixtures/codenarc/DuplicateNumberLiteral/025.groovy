
            @MyAnnotation1NumberValue(11)
            @MyAnnotation2NumberValue(11)
            @MyAnnotation2ListValue(value = [11, 12])
            class MyClass1 {
            }

            @MyAnnotation1ListValue(value = [11])
            @MyAnnotation2NumberValue(11)
            class MyClass2 {
            }

            @MyAnnotation1ListValue(value = [11, 12])
            @MyAnnotation2ListValue(value = [12, 11])
            class MyClass3 {
            }
        