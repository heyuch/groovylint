
            class MyClass {

                final lock = new java.util.concurrent.locks.ReentrantLock()

                def method() {
                    synchronized(lock) { }
                }
            }
        