
            class AbstractParent {
            }
            class BaseParent {
            }
            class MyClass {
                void method1() { }
                void method2() {  }
            }
            abstract class MyClass2 extends AbstractParent{
                void method1() { }
                void method2() {  }
            }
            abstract class MyClass3 extends BaseParent{
                void method1() { }
                void method2() {  }
            }
            abstract class MyBaseClass {
                void method1() { }
                abstract void method2()
            }

            interface MyMarkerInterface {
            } 