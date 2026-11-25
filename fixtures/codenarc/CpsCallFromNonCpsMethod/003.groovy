
            import org.codenarc.rule.jenkins.NonCPS
            
            class Main {
                Object script = new Object()
                
                Main(int i = getValueStatic()) {
                
                }
            
                static int getValueStatic() {
                    return 42
                }
                
                int getValue() {
                    return 42
                }
            
                @NonCPS
                void runSafe(int i = getValue(), 
                             int j = getValueStatic(), 
                             String s = script.sh('echo hello')) {
                    
                }
                
            }
        