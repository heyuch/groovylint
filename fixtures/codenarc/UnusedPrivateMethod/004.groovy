
            package groovy.bugs

            class MyClass {
                void myMethod() {
                    def x = MyInnerClass.&myInnerPrivateMethod1
                    def y = MyInnerClass.myInnerPrivateMethod2()
                }

                class MyInnerClass {
                    private static myInnerPrivateMethod1() { }
                    private static myInnerPrivateMethod2() { }
                    private static myInnerPrivateMethod3() { }
                    private static myInnerPrivateMethod4() { }
                }
            } 