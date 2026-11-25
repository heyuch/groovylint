
                def a =  {
                    return foo ? null : [] as String[]
                }
                def b =  {
                    return foo ? [] as String[] : null
                }
            