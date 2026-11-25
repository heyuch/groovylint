
            def defaultMethod = dc.metaClass.getMetaMethod(name, args)
            if(x == 99) {
                println "too much"
                if (y > 5){
                     println 'way too much'
                 }
             }
             return defaultMethod.invoke(delegate, args)
        