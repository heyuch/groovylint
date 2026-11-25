
            def x
            x = ready ? true : false
            x = doSomething(23) ? true : false
            x = null ? true : false
            x = 23 + 7 ? true : false
            x = y + 'x' ? true : false
            x = 'abc' =~ /./ ? true : false
            x = y <=> 99 ? true : false
        