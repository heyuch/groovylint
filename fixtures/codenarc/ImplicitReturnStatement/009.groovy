
            class MyClass {
                int execute() {
                    println 'execute'
                    for (int i=0; i<10; i++) {
                        if (value() > 1) {
                            return 99
                        }
                    }
                }
                int execute2() {
                    for(int i in [0, 1, 2]) {
                        println i
                    }
                }
            }
        