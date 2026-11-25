
        class MyClass {
            static {
                def e = '''Hello
                      ''' + 'world'   // should be joined
            }
        }
        