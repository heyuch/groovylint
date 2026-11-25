
            class MyTest extends GroovyTestCase {
                public void testMethod() {
                    MyOtherObject.helperMethod() // OK, not test
                    someTest.helperObject() // OK, not static
                    new MyOtherObject()
                }
            }
        