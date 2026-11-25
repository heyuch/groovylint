
            class MyClass {
                def byte4 = 1 as byte
                def short4 = 1 as short
                def double4 = 1 as double
                def integer4 = 1 as int
                def long4 = 1 as long
                def float4 = 1 as float
                def char4 = 1 as char

                def method() {
                    synchronized(byte4) {}
                    synchronized(short4) {}
                    synchronized(double4) {}
                    synchronized(integer4) {}
                    synchronized(long4) {}
                    synchronized(float4) {}
                    synchronized(char4) {}
                }
            }
        