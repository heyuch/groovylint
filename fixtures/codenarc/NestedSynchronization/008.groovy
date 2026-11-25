
            class NestedSynchronizationClass10 {
                private lock1 = new Object()
                private lock2 = new Object()

                def myMethod() {
                    synchronized(lock1) {
                        synchronized(lock2) {}
                    }
                }
            } 