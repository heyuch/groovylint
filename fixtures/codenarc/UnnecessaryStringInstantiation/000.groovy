
            class MyClass {
                def b1 = new String(xyz)
                def exception = new Exception('bad')
                def b2 = new String('abc')
                def b3 = new java.lang.String("abc")
                def b4 = new String('abc'.bytes)
                def b5 = new String([1,2,3] as char[])
            }
        