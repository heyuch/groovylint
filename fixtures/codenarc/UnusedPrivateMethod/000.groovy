
            class ExecuteTest {
                private String getCmd() { 'cmd' }
                private String getCmd2() { 'cmd' }
                private String getCMD() { 'cmd' }
                private String getCMD2() { 'cmd' }
                private void setCMD3(x) {  }
                private boolean isCmd4() { false }
                private boolean isCmd5() { true }

                private String getUnused() { 'unused' }

                void myMethod() {
                    cmd.execute(cmd2)
                    new Foo().CMD
                    foo.CMD2.bar = 'xxx'
                    CMD3 = 'yyy'
                    cmd4.class
                    cmd5.toString()
                }
            }
            