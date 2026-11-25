
            class MyClass {
                @Pointcut("@annotation(example.annotation.Test)")
                void myMethod1(String name) { }
            }
        