
             class MyTestCase extends TestCase {
                void testMethod() {
                   assertFalse(foo() != bar())
                   assertFalse('message', obj.foo() != obj.bar())
                }
              }
        