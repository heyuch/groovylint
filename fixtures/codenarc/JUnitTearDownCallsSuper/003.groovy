
          class MyTest extends TestCase {
            void tearDown() {
                println 'bad'
                super.someOtherMethod()
            }
          }
        