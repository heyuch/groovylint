
            class MyClass {
                String s
                MyClass() { s = '{"json": true}' }
                MyClass(@Annotation('${prop}') String s) { println 123 }
            }
        