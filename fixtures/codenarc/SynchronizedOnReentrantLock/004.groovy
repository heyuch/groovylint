
            class MyClass {
                // implicit typing
                final def lock = "stringLock" as ReentrantLock

                def method() {
                    // violation
                    synchronized(lock) { }
                }
            }
        