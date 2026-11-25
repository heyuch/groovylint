
            import org.codenarc.rule.jenkins.NonCPS
            
            class SomeClass {
                SomeClass() {}
                
                @NonCPS
                void safeMethod() {}
                
                @NonCPS
                static void staticSafeMethod() {}
            }
             
            class Main {
            
                @NonCPS
                void runSafe(def script) {
                    List l = [1,2,3]
                    l.get(0)
                    SomeClass s = new SomeClass()         
                    s.safeMethod()
                    SomeClass.staticSafeMethod()
                    s.equals(s)
                    
                    script.echo('hello world')
                    script.properties([])
                    script.getContext(hudson.model.Node)
                }
            }
        