
            class MyClass {
                Byte byte1 = 100
                Short short1 = 1
                Double double1 = 1
                Integer integer1 = 1
                Long long1 = 1
                Float float1 = 1
                Character char1 = 1

                def method() {
                    synchronized(byte1) {}
                    synchronized(short1) {}
                    synchronized(double1) {}
                    synchronized(integer1) {}
                    synchronized(long1) {}
                    synchronized(float1) {}
                    synchronized(char1) {}
                }
            }
        