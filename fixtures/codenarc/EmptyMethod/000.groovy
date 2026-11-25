
            class MyClass {
                @Override
                public void method1() {}

                public void method2() {
                    4 + 5
                }
            }
            abstract class MyBaseClass {
                // OK, handled by EmptyMethodInAbstractClass Rule
                public void method() {}
            }
        