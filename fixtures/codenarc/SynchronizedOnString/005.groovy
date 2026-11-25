
            class MyClass {
                // implicit typing
                final def lock = new Object[0] // correct idiom

                def method() {
                    return new Runnable() {
                        final def lock = "" // shadows parent from inner class
                        public void run() {
                            // violation
                            synchronized(lock) { }
                        }
                    }
                }
            }
        