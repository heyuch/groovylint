
          class MyTest extends TestCase {
            void setUp() {
                super.setUp()
                ant.delete(dir:appBase, failonerror:false)
            }
          }
        