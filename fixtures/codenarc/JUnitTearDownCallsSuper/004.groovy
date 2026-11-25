
          class MyTest extends TestCase {
            void tearDown() {
                println 'bad'
                other.tearDown()
            }
          }
        