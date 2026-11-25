
            import org.codenarc.rule.jenkins.NonCPS
            
            class SomeClass {
                public int value = getValueStatic()
                public int otherValue = Main.getValueStatic()
                
                @NonCPS
                static int getValueStatic() {
                    return 42
                }
                
            }
             
            class Main {
            
                @NonCPS
                static int getValueStatic() {
                    return 42
                }
            }
        