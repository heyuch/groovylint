
            package groovy.bugs

            class MyClass {
                void myMethod() {
                    MyClass.myPrivateMethod()
                    MyOuterClass.myInnerPrivateMethod()
                }
                private static void myPrivateMethod() {
                }
                private static void myUncalledPrivateMethod() {
                }
            }

            class MyOuterClass {
                private static myInnerPrivateMethod() {
                }
                private static myUnCalledPrivateMethod() {
                }
            }