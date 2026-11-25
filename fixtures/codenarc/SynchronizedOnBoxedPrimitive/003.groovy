
            class MyClass {
                def byte3 = new Byte((byte)100)
                def short3 = new Short((short)1)
                def double3 = new Double((double)1)
                def integer3 = new Integer(1)
                def long3 = new Long(1)
                def float3 = new Float(1)
                def char3 = new Character((char)'1')

                def method() {
                    synchronized(byte3) {}
                    synchronized(short3) {}
                    synchronized(double3) {}
                    synchronized(integer3) {}
                    synchronized(long3) {}
                    synchronized(float3) {}
                    synchronized(char3) {}
                }
            }
        