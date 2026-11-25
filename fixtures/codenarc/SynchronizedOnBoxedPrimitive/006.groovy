
            class MyClass {
                def byte7 = (byte)1
                def short7 = (short)1
                def double7 = (double)1
                def integer7 = (int)1
                def long7 = (long)1
                def float7 = (float)1
                def char7 = (char)1

                def method() {
                    synchronized(byte7) {}
                    synchronized(short7) {}
                    synchronized(double7) {}
                    synchronized(integer7) {}
                    synchronized(long7) {}
                    synchronized(float7) {}
                    synchronized(char7) {}
                }
            }
        