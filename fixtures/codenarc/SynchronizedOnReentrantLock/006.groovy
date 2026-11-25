
            class MyClass {
                // implicit typing
                final def lock = new Object[0] // correct idiom

                class MyInnerClass {

                    final lock = value as ReentrantLock // shadows parent from inner class
                    def method() {
                        // violation
                        synchronized(lock) { }
                    }
                }
            }
        