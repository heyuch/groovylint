
            class MyClass {
                def integer1 = 5I
                def integer2 = 5
                def long1 = 5L
                def double1 = 1d
                def float1 = 5F

                def method() {
                    synchronized(integer1) {}
                    synchronized(integer2) {}
                    synchronized(long1) {}
                    synchronized(double1) {}
                    synchronized(float1) {}
                }
            }
        