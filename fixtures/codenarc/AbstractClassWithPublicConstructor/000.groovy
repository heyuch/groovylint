
            class MyClass { }

            abstract class MyClass2 { }

            abstract class MyClass3 {
                protected MyClass3() { }
                def method1() { }
                public int method2() { }
            }

            abstract class MyClass4 extends AbstractParent {
                protected MyClass4() {
                    this(23)
                }
                private MyClass4(int count) { }
            }

            interface MyInterface { }
            