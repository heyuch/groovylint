
            @MyAnnotation1StringValue("string1")
            @MyAnnotation2StringValue("string1")
            @MyAnnotation2ListValue(value = ["string1", "string2"])
            class MyClass1 {
            }

            @MyAnnotation1ListValue(value = ["string1"])
            @MyAnnotation2StringValue("string1")
            class MyClass2 {
            }

            @MyAnnotation1ListValue(value = ["string1", "string2"])
            @MyAnnotation2ListValue(value = ["string2", "string1"])
            class MyClass3 {
            }
        