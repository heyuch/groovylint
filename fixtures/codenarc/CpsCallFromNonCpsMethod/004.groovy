
            import org.codenarc.rule.jenkins.NonCPS
            
            class Main {
                Object script = new Object()
                
                Main(int i = getValueStatic()) {
                
                }
            
                @NonCPS
                static int getValueStatic() {
                    return 42
                }
                
                @NonCPS
                int getValue() {
                    return 42
                }
                
                int getValueUnsafe() {
                    return 42
                }
            
                @NonCPS
                void runSafe(int i = getValue(),
                             int j = getValueStatic()) {
                    
                }
                
                void runUnsafe(int i = getValue(), 
                               int j = getValueUnsafe(), 
                               int k = getValueStatic(), 
                               String s = script.sh('echo hello')) {
                
                }
            }
        