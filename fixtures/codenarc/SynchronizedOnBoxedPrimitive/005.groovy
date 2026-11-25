
            class MyClass {
                def byte5 = 1 as Byte
                def short5 = 1 as Short
                def double5 = 1 as Double
                def integer5 = 1 as Integer
                def long5 = 1 as Long
                def float5 = 1 as Float
                def char5 = 1 as Character

                def method() {
                    synchronized(byte5) {}
                    synchronized(short5) {}
                    synchronized(double5) {}
                    synchronized(integer5) {}
                    synchronized(long5) {}
                    synchronized(float5) {}
                    synchronized(char5) {}
                }
            }
        