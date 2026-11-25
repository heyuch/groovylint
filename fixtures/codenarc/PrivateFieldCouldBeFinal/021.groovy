
            class MyClass {
                private int count = 0
                def runnable = new Runnable() {
                    void run() {
                        count = count + 5
                    }
                }
            }
        