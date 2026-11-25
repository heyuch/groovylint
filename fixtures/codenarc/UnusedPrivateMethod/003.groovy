
            class MyClass {
                void myMethod() {
                    def x = MyClass.&myPrivateMethod
                    def y = MyOuterClass.&myInnerPrivateMethod
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