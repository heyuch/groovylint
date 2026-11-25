
            class MyClass {

                final ReentrantLock lock = (ReentrantLock)"stringLock"

                class MyInnerClass {
                    def method() {
                        // violation
                        synchronized(lock) { }
                    }
                }
            }
        