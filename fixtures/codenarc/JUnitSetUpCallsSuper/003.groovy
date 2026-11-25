
          class MyTest extends TestCase {
            void setUp() {
                println 'bad'
                super.someOtherMethod()
            }
          }
        