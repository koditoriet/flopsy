#!/bin/sh
set -e
if [ "$PORT" != "" ] ; then
    BIND_OPT="-p $PORT"
fi
if [ "$MAX_BACKOFF" != "" ] ; then
    BACKOFF_OPT="-B $MAX_BACKOFF"
fi
if [ "$HOSTS" != "" ] ; then
    HOST_OPT="-H $HOSTS"
fi

CHECK_SCRIPT=/etc/flopsy/check-node.sh
if [ -f $CHECK_SCRIPT ] ; then
    if [ -x $CHECK_SCRIPT ] ; then
        CHECK_OPT="-c $CHECK_SCRIPT"
    else
        echo "'$CHECK_SCRIPT' ignored because is not executable."
    fi
fi

/usr/local/bin/flopsy $BIND_OPT $HOST_OPT $BACKOFF_OPT $CHECK_OPT -f /etc/flopsy/triggers.d