
            import org.junit.jupiter.api.Test
            class MyTestCase {
                void doSomething() { }          // not named test*()
                void testMe1(int count) { }     // not zero-arg
                int testMe2() { }               // not void
                private void testMe3() { }      // not public
                static void testMe4() { }       // static
            }
        