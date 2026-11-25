
            println 'w'
            println 'x'

            def y = 'y'
            def z = 'z'
            class MyClass {
                def static X = 'xyz'
                def static Y = 'xyz'
                def field = System.getProperty('file.seperator')
                def x = 'foo'
                def y = 'bar'
                String a = 'a'
                String b = 'b'
                def method() {
                    method('c', 'd')
                    ('e' == 'f') ? 'g' : 'h'
                    'i' ?: 'j'
                    return 'return'
                }
            }

            println 123
            println 123
        