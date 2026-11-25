
            class MyClass1 {
                final Object lock = new Object[0]
                def method() {
                    synchronized(lock) { }
                }

                class MyInnerClass {
                    def method() {
                        synchronized(lock) { }
                    }
                }
            }

            class MyClass2 {
                final def lock = new Object[0]
                def method() {
                    synchronized(lock) { }
                }
            }

            class MyClass3 {
                final def lock = new Object[0] // correct idiom
                def method() {
                    return new Runnable() {
                        public void run() {
                            synchronized(lock) { }
                        }
                    }
                }
            }

            class MyClass4 {
                final def lock = true

                class MyInnerClass {

                    final def lock = new Object[0] // shadows parent from inner class
                    def method() {
                        synchronized(lock) { }
                    }
                }
            }
        