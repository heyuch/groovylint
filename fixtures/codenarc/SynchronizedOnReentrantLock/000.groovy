
            class MyClass1 {
                final lock = new ReentrantLock()
                def method()  {
                   lock.lock()
                   try { } finally { lock.unlock() }
                }

                class MyInnerClass {
                    def method() {
                        lock.lock()
                        try { } finally { lock.unlock() }
                    }
                }
            }

            class MyClass2 {
                final lock = new ReentrantLock()
                def method() {
                   lock.lock()
                   try { } finally { lock.unlock() }
                }
            }

            class MyClass3 {
                final lock = new ReentrantLock()
                def method() {
                    return new Runnable() {
                        public void run() {
                           lock.lock()
                           try { } finally { lock.unlock() }
                        }
                    }
                }
            }

            class MyClass4 {
                final lock = new ReentrantLock()

                class MyInnerClass {

                    final def lock = new Object[0] // shadows parent from inner class
                    def method() {
                        synchronized(lock) { }
                    }
                }
            }
        