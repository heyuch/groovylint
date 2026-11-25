
            import org.codenarc.rule.jenkins.NonCPS
            
            class SomeClass implements Serializable {
                SomeClass() {}
                
                @NonCPS
                void safeMethod() {}
                
                @NonCPS
                static void staticSafeMethod() {}
                
                void unsafeMethod() {}
            }
             
            class Main {
                
                void runUnsafe() {
                    SomeClass s = new SomeClass()         
                    s.safeMethod()
                    SomeClass.staticSafeMethod()
                    s.equals(s)
                    
                    s.unsafeMethod()
                    
                    script.sh('echo')
                } 
            }
        