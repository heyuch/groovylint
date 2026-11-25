
            class MyClass {
                byte byte2 = getValue()
                short short2 = getValue()
                double double2 = getValue()
                int integer2 = getValue()
                long long2 = getValue()
                float float2 = getValue()
                char char2 = getValue()

                def method() {
                    synchronized(byte2) {}
                    synchronized(short2) {}
                    synchronized(double2) {}
                    synchronized(integer2) {}
                    synchronized(long2) {}
                    synchronized(float2) {}
                    synchronized(char2) {}
                }
            }
        