
            import org.codenarc.rule.jenkins.NonCPS
            
            class SomeClass {
            
                SomeClass() {
                    unsafeMethod()
                    unsafeStaticMethod()
                }
                
                void unsafeMethod() {}
                
                static void unsafeStaticMethod() {}
            }
             
            class Main {
            
                @NonCPS
                void runSafe() {
                    SomeClass s = new SomeClass()         
                    s.unsafeMethod()
                    SomeClass.unsafeStaticMethod()
                    def script = new Object()
                    script.sh("echo")
                }
            }
        