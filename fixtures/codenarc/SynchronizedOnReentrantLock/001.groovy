
            class MyClass5 {
                final ReentrantLock lock = value()
                def method() {
                    synchronized(lock) { }
                }
            }
        