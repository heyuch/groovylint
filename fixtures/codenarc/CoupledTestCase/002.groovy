
            class MyTest extends GroovyTestCase {
                public void testMethod() {
                    // violation, instantiation of another test class
                    new MyOtherTest()
                }
            }
        