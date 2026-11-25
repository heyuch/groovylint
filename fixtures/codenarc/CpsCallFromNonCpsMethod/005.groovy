
            
            class SomeClass {
                public int value = getValueStatic()
                public int otherValue = Main.getValueStatic()
                
                static int getValueStatic() {
                    return 42
                }
                
            }
             
            class Main {
            
                static int getValueStatic() {
                    return 42
                }
            }
        