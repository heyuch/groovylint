
            class MyClass {
                def myMethod() {
                    def closure = { }
                    if (true) { }
                    while(ready) { }
                    try {
                    } catch(Exception e) {
                    } finally { }
                    for(int i=0; i<10; i++) { }
                    for(String name in names) { }
                    for(String name: names) { }
                    if (count > this."maxPriority${priority}Violations") { }
                    while (count > this."maxPriority${priority}Violations") { }
                }
                MyClass() {
                    this(classNames)
                }
                MyClass(String s) { }
                MyClass(@Annotation('${prop}') String s) {
                }
                MyClass(Date date) { // comment
                    this(classNames)
                }
                MyClass(Object object) { /* comment */ }
            }
            interface MyInterface { }
            enum MyEnum { OK, BAD }
            trait MyTrait { }
        