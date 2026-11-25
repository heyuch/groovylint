
            class TestClass {
                void methodWithoutNestedForLoops() {
                    for (int i = 0; i < 100; ++i) {
                        println i
                    }
                    for (int i = 0; i < 200; ++i) {
                        println i
                    }
                }
            }
        