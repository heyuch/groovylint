
          class MyTest extends TestCase {
            void tearDown() {
                super.tearDown()
                println 'bad'
            }
          }
        